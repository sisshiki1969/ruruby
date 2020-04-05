f = Fiber.new do
  3.times {|x|
    #puts "#{x} #{self}"
    Fiber.yield x
  }
end

puts self
5.times do
  p f.resume
end