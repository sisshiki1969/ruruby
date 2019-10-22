class Vec
    @xxx=100
    def set_xxx(x)
        @xxx = x
    end
    def len(x,y)
        def sq(x)
            @xxx=1
            x*x
        end
        sq(x)+sq(y)
    end
    def get_xxx
        @xxx
    end
    def self.get_xxx
        @xxx = @xxx + 1
        @xxx
    end
end

foo1 = Vec.new
puts(foo1.len(3,4))
foo1.set_xxx(777)
puts(foo1.get_xxx)
foo2 = Vec.new
puts(foo2.set_xxx(999))
puts(foo1.get_xxx)
puts(foo2.get_xxx)
puts(Vec.new.get_xxx)
puts(Vec.get_xxx)
puts(Vec.get_xxx)
puts(Vec.get_xxx)