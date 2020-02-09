def func(y)
    [1,2,3,4].each do |x|
        return 100 if x == y
    end
    0
end

puts func(3)
puts func(7)