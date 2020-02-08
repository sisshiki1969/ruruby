def func(y)
    Proc.new{|x|
        puts x
        next 100 if x == y
        777
    }
end

puts func(3).call(4)