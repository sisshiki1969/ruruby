class Sub
    def self.main
        puts "success"
    end
end

Sub.main

class Main < Sub
end

Main.main