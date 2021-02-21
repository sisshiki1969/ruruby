class C
    def fn
        i = 0
        while i < 10000000 do
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            a = @a
            i += 1
        end
    end
end

o = C.new
o.fn