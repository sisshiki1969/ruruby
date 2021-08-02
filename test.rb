def f
  a = 0
  b = 1
  k = 1.times do
    v = 2
    a = 5
    break binding
  end
  b = 10
  k
end

b = f
puts b
p b.local_variables
eval "puts a", b
eval "puts b", b
eval "puts v", b
#puts b1.source_location