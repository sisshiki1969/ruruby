def iich(arr) # 引数に配列を取る
    idx = 0
    while idx < arr.size
      yield(arr[idx]) # 引数の各要素毎に、その要素を引数にしてブロックを起動
      idx += 1
    end
end

sum = 0
iich([1,2,3,4,5]) {|elem| p sum; sum += elem}
p sum