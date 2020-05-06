class Array
    def iich
        for i in 0...self.size
            puts "i:#{i}, ary:#{self[i].inspect} size:#{self.size}"
            yield self[i]
        end
    end
end

sum = 0
[3,4,5,6,7,8].iich{|x| puts sum; sum += x }
puts sum