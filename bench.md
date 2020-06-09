# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.79 ± 0.01 s | 2.84 ± 0.01 s | x 1.59 |
| app_mandelbrot.rb | 2.26 ± 0.03 s | 6.66 ± 0.22 s | x 2.95 |
| fibo.rb | 0.47 ± 0.02 s | 2.56 ± 0.07 s | x 5.39 |
| block.rb | 0.43 ± 0.01 s | 1.11 ± 0.03 s | x 2.56 |
| ao_bench.rb | 9.73 ± 0.16 s | 27.83 ± 0.58 s | x 2.86 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.8K | 4.8K | x 0.35 |
| app_mandelbrot.rb | 13.8K | 5.1K | x 0.37 |
| fibo.rb | 13.6K | 4.6K | x 0.34 |
| block.rb | 13.6K | 4.5K | x 0.33 |
| ao_bench.rb | 14.4K | 5.8K | x 0.40 |
