def assert(expected, actual)
    if expected == actual
        puts "OK: #{expected}"
    else
        puts "BAD: expected:#{expected} actual:#{actual}"
    end
end

class Vec
    @xxx=100
    def set_xxx(x); @xxx = x; end
    def len(x,y)
        def sq(x); x*x; end
        sq(x)+sq(y)
    end
    def get_xxx; @xxx; end
    def self.get_xxx; @xxx = @xxx + 1; @xxx; end
end

foo1 = Vec.new
assert(25, foo1.len(3,4))
foo1.set_xxx(777)
assert(777, foo1.get_xxx)
foo2 = Vec.new
foo2.set_xxx(999)
assert(777, foo1.get_xxx)
assert(999, foo2.get_xxx)
assert(nil, Vec.new.get_xxx)
assert(101, Vec.get_xxx)
assert(102, Vec.get_xxx)
assert(103, Vec.get_xxx)

