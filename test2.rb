f = Fiber.new do
  n = 0
  while true do
    Fiber.yield n
    n += 1
  end
end
  
3.times do
  p f.resume
end