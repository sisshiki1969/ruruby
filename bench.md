# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.35 s | 2.79 s | x 2.07 |
| app_mandelbrot.rb | 2.03 s | 8.28 s | x 4.08 |
| fibo.rb | 0.52 s | 2.63 s | x 5.06 |
| block.rb | 0.44 s | 0.95 s | x 2.16 |
| ao_bench.rb | 7.77 s | 34.53 s | x 4.44 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.14  K | 8.16  K | x 0.73 |
| app_mandelbrot.rb | 0.01  M | 1.72  M | x 148.60 |
| fibo.rb | 11.40  K | 2.08  K | x 0.18 |
| block.rb | 11.59  K | 2.05  K | x 0.18 |
| ao_bench.rb | 0.01  M | 4.40  M | x 366.31 |
