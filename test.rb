s = "outer"
k = Class.new{|c|
      puts self == c

      def initialize
        p "in initialize"
      end

      puts s

      def hoge
        p "hoge"
      end
    }
o = k.new              #=> "in initialize"
o.hoge                 #=> "hoge hoge hoge"