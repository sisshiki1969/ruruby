500000.times do
  f = Fiber.new { Fiber.yield }
  f.resume
end