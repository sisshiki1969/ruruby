f = []
1000.times {|x|
  f[x] = Fiber.new do
    1000.times {|n|
      Fiber.yield(x * n)
    }
  end
}

1000.times {|x|
  1000.times {
    f[x].resume
  }
}