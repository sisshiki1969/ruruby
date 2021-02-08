f = Fiber.new do
  loop { Fiber.yield }
end

10000000.times do |x|
  f.resume
end