class Vec
    def init(r,i)
        @r=r; @i=i
    end
    def put
        puts("Complex r=#{@r} i=#{@i}")
    end
    def get_r()
        @r
    end
    def get_i()
        @i
    end
    def add(v)
        res=Vec.new
        res.init(@r+v.get_r(), @i+v.get_i())
        res
    end
    def sq
        res=Vec.new
        res.init(@r*@r-@i*@i, 2*@r*@i)
        res
    end
end

v1 = Vec.new
v1.init(1.2,0.2)
for i in 0...10
    v1=v1.sq
    v1.put
end
