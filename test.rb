def func(y)
    [1,2,3,4].each{|x|
        puts x
        return 100 if x == y
    }
    200
end

puts func(3)