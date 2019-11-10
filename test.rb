class Foo
    def put
        puts "Foo"
    end
end

Bar = Foo
Foo = 4
Bar.new.put

class Bar
    def put
        puts "Boo"
    end
end

Bar.new.put
Foo.new.put
