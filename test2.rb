def calc(x,y)
    x * y
end

100_000.times do |x|
    1000.times do |y|
        calc(x,y)
    end
end
