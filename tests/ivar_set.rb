class C
    def fn
        i = 0
        while i < 10000000 do
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            @a = 100
            i += 1
        end
    end
end

o = C.new
o.fn