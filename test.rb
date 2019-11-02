def fibo(x)
    if x == 0 || x ==1 then x
    else fibo(x-1) + fibo(x-2)
    end
end

puts(fibo(35))