BOO = 100
class Foo
    FOO = 222
    puts 100, BOO
    def foo
        puts 333, ::Bar::BAR
    end
end
class Bar
    BAR = 333
    puts 100, BOO
    puts 222, ::Foo::FOO
end
Foo.new.foo