import numpy as np

class ASCFixedConst:
  def __init__(self, n, m, k, constants):
    self.n = n # number of multipliers
    self.m = m # fractional bits
    self.k = k # integral bits
    self.constant_array = constants

  def calculate_error(self):
    # Generate all possible binary values within the range
    square_mat_size = 2 **(self.m + self.k)
    multiplication_input = np.array([i * 2**-self.m for i in range(0, square_mat_size)]).reshape(-1, 1)
    multiplication_table = multiplication_input @ multiplication_input.T
    full_table = np.array([multiplication_table] * self.n)
    rounded_table = np.trunc(full_table + self.constant_array)
    return (np.sum(rounded_table - full_table)) / ((square_mat_size ** 2) * self.n)
  
class ASCNormal(ASCFixedConst):
  def generate_constants(self):
    normal = np.random.normal(0.5, 1/6, self.n) # Generate random constants under normal distribution of mean 0.5 and standard deviation 1/6
    normal[normal < 0] = 0
    normal[normal > 1] = 1
    normal = np.floor(normal * (2**(2*self.m))) / (2**(2*self.m)) # Quantize into 2m bits
    return np.repeat(normal, self.square_mat_size ** 2).reshape(self.n, self.square_mat_size, self.square_mat_size) # Repeat constants

  def __init__(self, n, m, k):
    self.n = n
    self.m = m
    self.k = k
    self.square_mat_size = 2 ** (self.m + self.k)
    constants = self.generate_constants()
    super().__init__(n, m, k, constants)

class ASCDegenerate(ASCFixedConst):
  def generate_constants(self):
    degenerate = np.array([0.5 for _ in range(self.n)]) # Generate a list of 0.5
    return np.repeat(degenerate, self.square_mat_size ** 2).reshape(self.n, self.square_mat_size, self.square_mat_size) # Repeat constants

  def __init__(self, n, m, k):
    self.n = n
    self.m = m
    self.k = k
    self.square_mat_size = 2 ** (self.m + self.k)
    constants = self.generate_constants()
    super().__init__(n, m, k, constants)

class ASCUniform(ASCFixedConst):
  def generate_constants(self):
    assert self.n % self.square_mat_size == 0, f"n must be a multiple of square_mat_size, {self.square_mat_size}" # Ensure n is a multiple of square_mat_size
    
    uniform = np.array([i * 2**-(2*self.m) for i in range(0, 2**(2*self.m))]) # Generate a list of uniform constants
    return np.repeat(uniform, (self.n//self.square_mat_size)*self.square_mat_size**2//(2**abs(self.m-self.k))).reshape(self.n, self.square_mat_size, self.square_mat_size) # Repeat constants

  def __init__(self, n, m, k):
    self.n = n
    self.m = m
    self.k = k
    self.square_mat_size = 2 ** (self.m + self.k)
    constants = self.generate_constants()
    super().__init__(n, m, k, constants)


class ASCRandom(ASCFixedConst):
  def generate_constants(self):
    random = np.floor(np.random.uniform(0, 1, self.n)*2**(2*self.m))/(2**(2*self.m)) # Generate random constants under uniform distribution between 0 and 1, quantizing them into 2m bits
    return np.repeat(random, self.square_mat_size ** 2).reshape(self.n, self.square_mat_size, self.square_mat_size) # Repeat constants

  def __init__(self, n, m, k):
    self.n = n
    self.m = m
    self.k = k
    self.square_mat_size = 2 ** (self.m + self.k)
    constants = self.generate_constants()
    super().__init__(n, m, k, constants)