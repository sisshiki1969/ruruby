class Array
    def map(fun)
        a = []
        for i in 0...self.length
            a.push fun.call(self[i])
        end
        a
    end
end

a = 3
puts ([1,2,3,4].map(-> x { x*x*a }))

a = 5
puts ([1,2,3,4].map(-> x { x*x*a }))