a = Fiber.new {
  1000_000.times {|x|
    Fiber.yield x
  }
}

1000_000.times {
  a.resume
}