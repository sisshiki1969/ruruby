def fibo(x)
    if x < 2 then x
    else fibo(x-1) + fibo(x-2)
    end
end

puts(fibo(28))