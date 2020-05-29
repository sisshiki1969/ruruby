# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.30 ± 0.01 s | 2.58 ± 0.02 s | x 1.99 |
| app_mandelbrot.rb | 1.95 ± 0.01 s | 9.11 ± 0.08 s | x 4.67 |
| fibo.rb | 0.50 ± 0.02 s | 2.88 ± 0.04 s | x 5.76 |
| block.rb | 0.43 ± 0.01 s | 1.06 ± 0.03 s | x 2.50 |
| ao_bench.rb | 7.68 ± 0.05 s | 36.35 ± 0.34 s | x 4.73 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.2K | 5.7K | x 0.51 |
| app_mandelbrot.rb | 11.1K | 30.2K | x 2.71 |
| fibo.rb | 11.1K | 2.1K | x 0.19 |
| block.rb | 11.1K | 2.1K | x 0.19 |
| ao_bench.rb | 11.5K | 58.6K | x 5.11 |
