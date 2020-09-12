    fib = Fiber.new do
        Fiber.yield a=b=1
        loop { 
            a,b=b,a+b
            Fiber.yield a
        }
    end

    res = *(0..7).map {
        fib.resume
    }

    p res