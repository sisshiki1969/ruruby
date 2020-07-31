10000.times do |x|
  f = Fiber.new { Fiber.yield([x.to_s] * 10000) }
  # f.resume
  # f.resume
end
GC.print_mark
