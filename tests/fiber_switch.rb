f = Fiber.new do
  loop { Fiber.yield }
end

20000000.times do |x|
  f.resume
end
