f = Fiber.new do
    n = 0
    while true do
      Fiber.yield n if n % 2 == 0
      Fiber.yield true
      n += 1
    end
  end
  
  5.times do
   p f.resume
  end