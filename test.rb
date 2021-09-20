def f(x)
    Proc.new {|z| puts x * z}
end

f(10).call(5)