def func
    a = 77 
    1.times {
        1.times {
            return Proc.new{
                puts a
                a = a + 1
            }
        }
    }
end

f = func
f.call
f.call
f.call
func.call