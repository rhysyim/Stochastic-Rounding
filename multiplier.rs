use std::fmt::Debug;
use std::cmp::Ordering;

use crate::utils::Wrap;

use derive_syn_parse::Parse;
use proc_macro::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, Token};
use proc_macro2::{Ident, TokenStream as TokenStream2, Literal, TokenTree};

#[derive(Clone, Copy, Debug)]
struct VerilogIdx(usize);
impl ToTokens for VerilogIdx {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.append(TokenTree::Literal(Literal::usize_unsuffixed(self.0)));
    }
}
impl From<usize> for VerilogIdx {
    fn from(src: usize) -> Self {
        Self(src)
    }
}

#[derive(Clone, Copy, Debug)]
struct PartialProduct {
    a: usize,
    b: usize
}
impl ToTokens for PartialProduct {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let a = self.a;
        let b = self.b;
        quote!{(self.a.val().get_bit(#a) & self.b.val().get_bit(#b))}.to_tokens(tokens)
    }
}
#[derive(Clone, Copy, Debug)]
enum ConstFMABit {
    Constant(bool),
    PartialProduct(PartialProduct),
}
impl ToTokens for ConstFMABit {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ConstFMABit::Constant(v) => quote!{#v}.to_tokens(tokens),
            ConstFMABit::PartialProduct(pp) => pp.to_tokens(tokens),
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum FMABit {
    Add(usize),
    PartialProduct(PartialProduct),
}
impl ToTokens for FMABit {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            FMABit::Add(idx) => quote!{self.offset.val().get_bit(#idx)}.to_tokens(tokens),
            FMABit::PartialProduct(pp) => pp.to_tokens(tokens),
        }
    }
}

#[derive(Clone, Copy)]
struct HalfAdder<T> {
    a: DaddaBit<T>,
    b: DaddaBit<T>,
}
#[derive(Clone, Copy)]
struct FullAdder<T> {
    a: DaddaBit<T>,
    b: DaddaBit<T>,
    c: DaddaBit<T>,
}
#[derive(Clone, Copy, Debug)]
enum DaddaBit<T> {
    Input(T),
    HalfAddSum(VerilogIdx),
    HalfAddCarry(VerilogIdx),
    FullAddSum(VerilogIdx),
    FullAddCarry(VerilogIdx),
    Constant(bool),
}
impl<T: ToTokens> ToTokens for DaddaBit<T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            DaddaBit::Input(t) => t.to_tokens(tokens),
            DaddaBit::Constant(b) => quote! {#b}.to_tokens(tokens),
            DaddaBit::HalfAddSum(idx) => quote! {self.half_adders[#idx].sum.val()}.to_tokens(tokens),
            DaddaBit::HalfAddCarry(idx) => quote! {self.half_adders[#idx].carry.val()}.to_tokens(tokens),
            DaddaBit::FullAddSum(idx) => quote! {self.full_adders[#idx].sum.val()}.to_tokens(tokens),
            DaddaBit::FullAddCarry(idx) => quote! {self.full_adders[#idx].carry_out.val()}.to_tokens(tokens),
        }
    }
}
struct DaddaResult<T> {
    bits: Vec<[DaddaBit<T>; 2]>,
    half_adders: Vec<HalfAdder<T>>,
    full_adders: Vec<FullAdder<T>>,
}

fn dadda<'a, T: Debug>(input: &'a [Vec<T>]) -> DaddaResult<&'a T> {
    let max_height = input.iter().map(|v| v.len()).max().unwrap();
    let mut iterations = vec![2];
    loop {
        let previous = iterations.last().unwrap();
        let next = previous + previous / 2;
        if next < max_height {
            iterations.push(next);
        } else {
            break;
        }
    }

    let output_width = input.len();

    let mut state = input
        .iter()
        .map(|v| v.iter().map(DaddaBit::Input).collect())
        .collect::<Vec<Vec<DaddaBit<&'a T>>>>();

    let mut half_adders = Vec::new();
    let mut full_adders = Vec::new();
    while let Some(max_height) = iterations.pop() {
        loop {
            let mut changed = false;
            for i in 0..(output_width - 1) {
                match state[i].len().cmp(&(max_height + 1)) {
                    Ordering::Less => {},
                    Ordering::Equal => {
                        let a = state[i].pop().unwrap();
                        let b = state[i].pop().unwrap();

                        let half_adder = HalfAdder { a, b };
                        let idx = half_adders.len();
                        state[i].insert(0, DaddaBit::HalfAddSum(idx.into()));
                        state[i + 1].insert(0, DaddaBit::HalfAddCarry(idx.into()));
                        half_adders.push(half_adder);
                        changed = true;
                    },
                    Ordering::Greater => {
                        let a = state[i].pop().unwrap();
                        let b = state[i].pop().unwrap();
                        let c = state[i].pop().unwrap();

                        let full_adder = FullAdder { a, b, c };
                        let idx = full_adders.len();
                        state[i].insert(0, DaddaBit::FullAddSum(idx.into()));
                        state[i + 1].insert(0, DaddaBit::FullAddCarry(idx.into()));
                        full_adders.push(full_adder);
                        changed = true;
                    },
                }
            }
            if !changed {
                break;
            }
        }
    }

    let mut bits = Vec::with_capacity(state.len());
    //println!("{:?}", state);
    for s in state {
        match s[..] {
            [a, b] => bits.push([a, b]),
            [a] => bits.push([a, DaddaBit::Constant(false)]),
            _ => unreachable!(),
        };
    }
    DaddaResult {
        bits,
        half_adders,
        full_adders,
    }
}
fn reduce<T: ToTokens + Debug>(bits: &[Vec<T>]) -> (TokenStream2, TokenStream2, TokenStream2) {
    let DaddaResult {
        bits,
        half_adders,
        full_adders,
    } = dadda(bits);

    let output_bits = bits.len();

    let num_half_adders = half_adders.len();
    let num_full_adders = full_adders.len();

    let ha_inputs = half_adders.iter().enumerate().map(|(idx, ha)| {
        let idx = VerilogIdx(idx);
        let a = ha.a;
        let b = ha.b;
        quote! {
            self.half_adders[#idx].a.next = #a;
            self.half_adders[#idx].b.next = #b;
        }
    });
    let fa_inputs = full_adders.iter().enumerate().map(|(idx, fa)| {
        let idx = VerilogIdx(idx);
        let a = fa.a;
        let b = fa.b;
        let carry_in = fa.c;
        quote! {
            self.full_adders[#idx].a.next = #a;
            self.full_adders[#idx].b.next = #b;
            self.full_adders[#idx].carry_in.next = #carry_in;
        }
    });
    let product_inputs = {
        let top_bits = bits
            .iter()
            .map(|column| {
                if column.is_empty() {
                    DaddaBit::Constant(false)
                } else {
                    column[0]
                }
            })
            .collect::<Vec<_>>();
        let bottom_bits = bits
            .iter()
            .map(|column| {
                if column.len() == 2 {
                    column[1]
                } else {
                    DaddaBit::Constant(false)
                }
            })
            .collect::<Vec<_>>();
        let indices_top = 0..output_bits;
        let indices_bottom = indices_top.clone();
        quote! {
            self.product_top.next = bits::<#output_bits>(#((#bottom_bits as u64) << #indices_top)|*);
            self.product_bottom.next = bits::<#output_bits>(#((#top_bits as u64) << #indices_bottom)|*);
        }
    };

    (
        quote! {
            half_adders: [HalfAdder<Bit>; #num_half_adders],
            full_adders: [FullAdder<Bit>; #num_full_adders],
            product_top: Signal<Local, Bits<#output_bits>>,
            product_bottom: Signal<Local, Bits<#output_bits>>,
            pub product: Signal<Out, Bits<#output_bits>>,
        },
        quote! {
            half_adders: std::array::from_fn::<HalfAdder<Bit>, #num_half_adders, _>(|_| Default::default()),
            full_adders: std::array::from_fn::<FullAdder<Bit>, #num_full_adders, _>(|_| Default::default()),
            product_top: Default::default(),
            product_bottom: Default::default(),
            product: Default::default(),
        },
        quote! {
            #(#ha_inputs)*
            #(#fa_inputs)*
            #product_inputs
            self.product.next = self.product_top.val() + self.product_bottom.val();
        }
    )
}

pub fn const_fma_impl(args: TokenStream) -> TokenStream {
    #[derive(Parse)]
    struct Args {
        ident: Ident,
        _0: Token![,],
        int_bits: Wrap<usize>,
        _1: Token![,],
        frac_bits: Wrap<usize>,
        _2: Token![,],
        constant: Wrap<usize>,
    }
    let Args {
        ident,
        int_bits: Wrap(int_bits),
        frac_bits: Wrap(frac_bits),
        constant: Wrap(constant),
        ..
    } = parse_macro_input!(args as Args);

    let input_bits = int_bits + frac_bits;
    let output_bits = input_bits * 2;
    let output_frac_bits = frac_bits * 2;

    let mut bits: Vec<Vec<ConstFMABit>> = Vec::with_capacity(output_bits);
    for _ in 0..output_bits {
        bits.push(Vec::new());
    }
    for a in 0..input_bits {
        for b in 0..input_bits {
            bits[a + b].push(ConstFMABit::PartialProduct(PartialProduct{a, b}));
        }
    }
    for (idx, col) in bits.iter_mut().enumerate().take(output_frac_bits) {
        col.push(ConstFMABit::Constant((constant >> idx) & 1 != 0));
    }

    let (
        reducer_types,
        reducer_defaults,
        reducer_update,
    ) = reduce(bits.as_slice());

    quote! {
        use rust_hdl::prelude::*;// unfortunately #[hdl_gen] is unhygenic

        #[derive(LogicBlock)]
        struct #ident {
            pub a: Signal<In, Bits<#input_bits>>,
            pub b: Signal<In, Bits<#input_bits>>,
            #reducer_types
            // I don't use this but rust_hdl wants it to exist
            pub clock: Signal<In, Clock>,
        }
        impl Default for #ident {
            fn default() -> Self {
                Self {
                    a: Default::default(),
                    b: Default::default(),
                    #reducer_defaults
                    clock: Default::default(),
                }
            }
        }
        impl Logic for #ident {
            #[::rust_hdl::prelude::hdl_gen]
            fn update(&mut self) {
                #reducer_update
            }
        }
    }
    .into()
}

pub fn fma_impl(args: TokenStream) -> TokenStream {
    #[derive(Parse)]
    struct Args {
        ident: Ident,
        _0: Token![,],
        int_bits: Wrap<usize>,
        _1: Token![,],
        frac_bits: Wrap<usize>,
    }
    let Args {
        ident,
        int_bits: Wrap(int_bits),
        frac_bits: Wrap(frac_bits),
        ..
    } = parse_macro_input!(args as Args);

    let input_bits = int_bits + frac_bits;
    let output_bits = input_bits * 2;
    let output_frac_bits = frac_bits * 2;

    let mut bits: Vec<Vec<FMABit>> = Vec::with_capacity(output_bits);
    for _ in 0..output_bits {
        bits.push(Vec::new());
    }
    for a in 0..input_bits {
        for b in 0..input_bits {
            bits[a + b].push(FMABit::PartialProduct(PartialProduct{a, b}));
        }
    }
    for (idx, col) in bits.iter_mut().enumerate().take(output_frac_bits) {
        col.push(FMABit::Add(idx));
    }

    let (
        reducer_types,
        reducer_defaults,
        reducer_update,
    ) = reduce(bits.as_slice());

    quote! {
        use rust_hdl::prelude::*;

        #[derive(LogicBlock)]
        struct #ident {
            pub a: Signal<In, Bits<#input_bits>>,
            pub b: Signal<In, Bits<#input_bits>>,
            pub offset: Signal<In, Bits<#output_frac_bits>>,
            #reducer_types
            pub clock: Signal<In, Clock>,
        }
        impl Default for #ident {
            fn default() -> Self {
                Self {
                    a: Default::default(),
                    b: Default::default(),
                    offset: Default::default(),
                    #reducer_defaults
                    clock: Default::default(),
                }
            }
        }
        impl Logic for #ident {
            #[hdl_gen]
            fn update(&mut self) {
                #reducer_update
            }
        }
    }.into()
}
