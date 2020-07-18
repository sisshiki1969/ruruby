#  The Computer Language Benchmarks Game
#  http://shootout.alioth.debian.org/
#
#  contributed by Karl von Laudermann
#  modified by Jeremy Echols

class Complex_
  attr_accessor :r, :i
  def initialize(r,i)
    @r=r
    @i=i
  end
  def *(other)
    Complex_.new(@r*other.r - @i*other.i, @r*other.i + @i*other.r)
  end
  def +(other)
    Complex_.new(@r + other.r, @i + other.i)
  end
  def abs2
    @r*@r + @i*@i
  end
end

size = 400 # ARGV[0].to_i

puts "P4\n#{size} #{size}"

def mandelbrot?(z, c)
  50.times do
    z = z * z + c
    return 0 if z.abs2 > 4.0
  end
  1
end

byte_acc = 0
bit_num = 0

# For..in loops are faster than .upto, .downto, .times, etc.
size.times do |y|
  size.times do |x|
    z = Complex_.new(0.0, 0.0)
    c = Complex_.new(2.0*x/size-1.5, 2.0*y/size-1.0)
    # To make use of the for..in code, we use a dummy variable,
    # like one would in C

    byte_acc = (byte_acc << 1) | mandelbrot?(z,c)
    bit_num += 1

    # Code is very similar for these cases, but using separate blocks
    # ensures we skip the shifting when it's unnecessary, which is most cases.
    if bit_num == 8
      print byte_acc.chr
      byte_acc = 0
      bit_num = 0
    elsif x == size - 1
      byte_acc = byte_acc << (8 - bit_num)
      print byte_acc.chr
      byte_acc = 0
      bit_num = 0
    end
  end
end
