f = Fiber.new do
  3.times {|x|
    Fiber.yield x
    sed
  }
end

5.times do
  p f.resume
end