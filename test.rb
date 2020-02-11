class Foo
    Bar = 100
    Ker = 777
end

class Bar < Foo
    Doo = 555
end

p Foo.const_get(:Bar)
p Bar.const_get(:Bar)
p Foo.constants
p Bar.constants