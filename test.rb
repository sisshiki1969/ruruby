class Array
    def iich
        len = self.size
        for i in 0...len
            puts self[i]
            yield self[i]
        end
    end
end

sum = 0
[1,2,3,4,5].iich{|x| sum += x }
puts sum
