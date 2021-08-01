def f
  res = []
  a = 0
  res << binding
  a = 5
  res << binding
  res
end

b1, b2 = f

eval "puts a", b1, __FILE__, __LINE__
eval "puts a", b2, __FILE__, __LINE__
puts b1.source_location
puts b2.source_location