class Foo
    attr_accessor :a
    def initialize
        @a = 0
    end
    def add_a(x)
        ->{@a = @a + x; puts @a}
    end
end

f = Foo.new
adda100 = f.add_a(100)
adda77 = f.add_a(77)
adda100.call
adda77.call
adda77.call
