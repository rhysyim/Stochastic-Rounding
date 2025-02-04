use derive_syn_parse::Parse;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, Token};

use crate::utils::Wrap;

pub fn lfsr_impl(args: TokenStream) -> TokenStream {
    #[derive(Parse)]
    struct Args {
        ident: Ident,
        _0: Token![,],
        width: Wrap<usize>,
    }
    let Args {
        ident,
        width: Wrap(width),
        ..
    } = parse_macro_input!(args as Args);

    let width_minus_1 = width - 1;
    quote! {
        use rust_hdl::prelude::*;
        #[derive(LogicBlock)]
        struct #ident {
            feedback: DFFWithInit<Bits<#width>>,
            pub clock: Signal<In, Clock>,
            pub out: Signal<Out, Bits<#width_minus_1>>,
        }
        impl Default for #ident {
            fn default() -> Self {
                Self {
                    feedback: DFFWithInit::new(1.into()),
                    clock: Signal::default(),
                    out: Signal::new_with_default(1.into()),// not sure why rust analyzer shits itself over this,
                    // because rustc is totally fine with it
                }
            }
        }
        impl Logic for #ident {
            #[hdl_gen]
            fn update(&mut self) {
                self.out.next = bit_cast::<#width_minus_1, #width>(self.feedback.q.val());
                self.feedback.d.next = (self.feedback.q.val().get_bit(0) as u64) << #width_minus_1 as u64
                    | bit_cast::<#width, #width_minus_1>(
                        bit_cast::<#width_minus_1, #width>(self.feedback.q.val())
                        ^ bit_cast::<#width_minus_1, #width>(self.feedback.q.val() >> 1)
                    );
                clock!(self, clock, feedback)

            }
        }
    }.into()
}
