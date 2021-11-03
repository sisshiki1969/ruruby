f = Fiber.new do
  loop { Fiber.yield }
end

5000000.times do |x|
  f.resume
end