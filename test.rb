class Vec
    def initialize(x,y)
        @x=x;@y=y
    end
    def add(v)
        Vec.new(@x + v.x, @y + v.y)
    end
    def x; @x; end
    def y; @y; end
end

v = Vec.new
puts("#{v.x} #{v.y}")
v1 = Vec.new(3,5)
puts("#{v1.x} #{v1.y}")
v2 = v1.add(Vec.new(4,8))
puts("#{v2.x} #{v2.y}")