def func
end

m1 = method(:func)
m2 = method(:func)

h = {}
h[m1] = 100
p h[m2]
puts ARGV