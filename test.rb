def inc
    a = 100
    ->{a = a + 1; puts a}
end

inc.call
inc.call
inc.call

p = inc()
p.call
p.call
p.call