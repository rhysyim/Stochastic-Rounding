import numpy as np

class ASC:
  def __init__(self, n, m, k):
    self.n = n # number of multipliers
    self.m = m # fractional bits
    self.k = k # integral bits
    self.multiplication_input = np.array([i * 2**-self.m for i in range(0, 2 **(self.m + self.k))]).reshape(-1, 1)
    self.simulated_input_space = self.multiplication_input.reshape(-1)

  def generate_constants(self):
    return np.array([0.5 for _ in range(self.n)])

  def calculate_error(self):
    square_mat_size = 2 **(self.m + self.k)
    multiplication_table = self.multiplication_input @ self.multiplication_input.T # Generate multiplication table
    full_table = np.array([multiplication_table] * self.n)
    constant_array = np.repeat(self.generate_constants(), square_mat_size ** 2).reshape(self.n, square_mat_size, square_mat_size)
    rounded_table = np.trunc(full_table + constant_array) # Apply rounding constants
    return (np.sum(rounded_table - full_table)) / ((square_mat_size ** 2) * self.n) # Calculate error

  def simulate_error(self):
    # Randomly generate two lists of length n
    input_1 = np.random.choice(self.simulated_input_space, (self.n))
    input_2 = np.random.choice(self.simulated_input_space, (self.n))
    mult = input_1 * input_2 # Pairwise multiplication between the two lists
    constant_array = self.generate_constants()
    rounded_array = np.trunc(constant_array + mult)
    total = np.sum(rounded_array - mult) / self.n # Calculate simulation error
    return total
  
class ASCCycles(ASC):
  def __init__(self, n, m, k, iterations_per_carray):
    super().__init__(n, m, k)
    self.iterations_per_carray = iterations_per_carray
    self.iterations = 0

  def generate_constants(self):
    if self.iterations % self.iterations_per_carray == 0:
      self.carray = np.random.uniform(0, 1, (self.n)) # Generate a new list of random constants every iterations_per_carray iterations

    self.iterations += 1
    return self.carray

class ASCShifter(ASC):
  def __init__(self, n, m, k):
    super().__init__(n, m, k)
    self.carray = np.random.uniform(0, 1, (n))

  def generate_constants(self):
    self.carray = np.roll(self.carray, 1) # Shift the list of random constants by 1
    self.carray[0] = np.random.uniform(0, 1)

    return self.carray
  
class ASCBroadcast(ASC):
  def __init__(self, n, m, k, num_constants):
    # num_constants is the number of random constants generated
    super().__init__(n, m, k)
    self.num_constants = num_constants

  def generate_constants(self):
    self.carray = np.tile(np.random.uniform(0, 1, (self.num_constants)), (self.n // self.num_constants + 1))[:self.n]
    return self.carray
  
class ASCUCyclesWithShuffle(ASC):
  def __init__(self, n, m, k, iterations_per_carray):
    super().__init__(n, m, k)
    self.iterations_per_carray = iterations_per_carray
    self.iterations = 0

  def generate_constants(self):
    if self.iterations % self.iterations_per_carray == 0:
      self.carray = np.random.uniform(0, 1, (self.n)) # Generate a new list of random constants every iterations_per_carray iterations
    else:
      np.random.shuffle(self.carray) # Shuffle list of random constants
    self.iterations += 1
    return self.carray