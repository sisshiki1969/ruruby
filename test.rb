def inc
    a = 100
    ->{a = a + 1; puts a}
end

p = inc()
p.call
p.call