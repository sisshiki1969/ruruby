def fibo(x)
    if x < 3
        1
    else
        fibo(x-1) + fibo(x-2)
    end
end

puts(fibo(28))