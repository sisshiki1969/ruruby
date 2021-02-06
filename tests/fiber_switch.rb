f = Fiber.new do
  loop { Fiber.yield }
end

200000.times do |x|
  f.resume
end