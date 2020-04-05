def enum2gen(enum)
  Fiber.new do
    3.times { |x|
      Fiber.yield(x)
    }
    #enum.each{|i|
    #  puts i
    #  Fiber.yield(i)
    #}
  end
end

g = enum2gen(1..100)

5.times do
  p g.resume
end