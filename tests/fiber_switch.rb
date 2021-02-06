f = Fiber.new do
  loop { Fiber.yield }
end

2000000.times do |x|
  f.resume
end