# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.32 ± 0.01 s | 2.49 ± 0.09 s | x 1.88 |
| app_mandelbrot.rb | 1.94 ± 0.02 s | 8.20 ± 0.06 s | x 4.22 |
| fibo.rb | 0.50 ± 0.01 s | 2.50 ± 0.04 s | x 5.03 |
| block.rb | 0.43 ± 0.01 s | 0.91 ± 0.04 s | x 2.12 |
| ao_bench.rb | 7.58 ± 0.04 s | 36.87 ± 3.47 s | x 4.86 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.6K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.4K | 3.5K | x 0.31 |
| fibo.rb | 11.0K | 2.1K | x 0.19 |
| block.rb | 11.3K | 2.0K | x 0.18 |
| ao_bench.rb | 11.5K | 4.1K | x 0.36 |
