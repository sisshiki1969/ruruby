class Foo
    def func(a,b)
        puts "hi!"
        p a,b
    end
end

class Bar < Foo
    def func(a,b)
        puts "ho!"
        super
    end
end

Foo.new.func(3,4)

Bar.new.func(3,4)
