#  The Computer Language Benchmarks Game
#  http://shootout.alioth.debian.org/
#
#  contributed by Karl von Laudermann
#  modified by Jeremy Echols

size = 600 # ARGV[0].to_i

puts "P4\n#{size} #{size}"

LIMIT_SQUARED = 4.0                 # Presquared limit

byte_acc = 0
bit_num = 0

# For..in loops are faster than .upto, .downto, .times, etc.
for y in 0...size
  for x in 0...size
    zr, zi = 0.0, 0.0
    cr, ci = 2.0*x/size - 1.5, 2.0*y/size - 1.0
    escape = false
    # To make use of the for..in code, we use a dummy variable,
    # like one would in C
    for dummy in 0...50
      tr, ti = zr*zr - zi*zi + cr, 2*zr*zi + ci
      zr, zi = tr, ti

      if zr*zr+zi*zi > LIMIT_SQUARED
        escape = true
        break
      end
    end

    byte_acc = (byte_acc << 1) | (escape ? 0 : 1)
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