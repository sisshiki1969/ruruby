#  The Computer Language Benchmarks Game
#  http://shootout.alioth.debian.org/
#
#  contributed by Karl von Laudermann
#  modified by Jeremy Echols

class Complexe
  attr_accessor(:r, :i)
  def initialize(r,i)
    @r=r; @i=i;
  end
  def *(other)
    Complexe.new(@r*other.r - @i*other.i, @r*other.i + @i*other.r)
  end
  def +(other)
    Complexe.new(@r + other.r, @i + other.i)
  end
  def abs2; @r*@r + @i*@i; end
end

size = 200 # ARGV[0].to_i

puts("P4\n#{size} #{size}")

ITER = 49                           # Iterations - 1 for easy for..in looping
LIMIT_SQUARED = 4.0                 # Presquared limit

byte_acc = 0
bit_num = 0

count_size = size - 1               # Precomputed size for easy for..in looping

# For..in loops are faster than .upto, .downto, .times, etc.
for y in 0..count_size
  for x in 0..count_size
    z = Complexe.new(0.0, 0.0)
    c = Complexe.new(2.0*x/size-1.5, 2.0*y/size-1.0)
    escape = false
    # To make use of the for..in code, we use a dummy variable,
    # like one would in C
    for dummy in 0..ITER
      z = z * z + c
      if z.abs2 > LIMIT_SQUARED
        escape = true
        break
      end
    end

    byte_acc = (byte_acc << 1) | (escape ? 0 : 1)
    bit_num += 1

    # Code is very similar for these cases, but using separate blocks
    # ensures we skip the shifting when it's unnecessary, which is most cases.
    if bit_num == 8
      print(byte_acc.chr)
      byte_acc = 0
      bit_num = 0
    elsif x == count_size
      byte_acc = byte_acc << (8 - bit_num)
      print(byte_acc.chr)
      byte_acc = 0
      bit_num = 0
    end
  end
end
