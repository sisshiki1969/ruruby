class Vec
    def len(x,y)
        def sq(x)
            x*x
        end
        sq(x)+sq(y)
    end
end

Vec.new.len(3,4)