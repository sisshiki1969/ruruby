# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.39 s | 3.18 s | x 2.29 |
| app_mandelbrot.rb | 1.99 s | 8.06 s | x 4.05 |
| fibo.rb | 0.51 s | 2.58 s | x 5.06 |
| block.rb | 0.44 s | 0.9 s | x 2.05 |
| ao_bench.rb | 7.86 s | 33.5 s | x 4.26 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.68  K | 8.25  K | x 0.71 |
| app_mandelbrot.rb | 0.01  M | 1.71  M | x 146.08 |
| fibo.rb | 11.38  K | 2.06  K | x 0.18 |
| block.rb | 11.35  K | 2.00  K | x 0.18 |
| ao_bench.rb | 0.01  M | 4.39  M | x 357.88 |
