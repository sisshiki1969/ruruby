x=47
y=125

zr = 0.0
zi = 0.0
cr = (2.0*x/600)-1.5
ci = (2.0*y/600)-1.0
escape = false

tr = 0

# To make use of the for..in code, we use a dummy variable,
# like one would in C
for dummy in 0..49
  puts(dummy)
  puts(tr)
  tr = zr*zr - zi*zi + cr
  puts(tr)
  ti = 2*zr*zi + ci
  puts(tr)
  zr = tr
  puts(tr)
  zi = ti
  if (zr*zr+zi*zi) > 4.0
    escape = true
    break
  end
end

puts(tr)
