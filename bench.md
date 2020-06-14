# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.73 ± 0.04 s | 2.91 ± 0.05 s | x 1.68 |
| app_mandelbrot.rb | 2.17 ± 0.02 s | 7.79 ± 0.15 s | x 3.59 |
| fibo.rb | 0.50 ± 0.03 s | 2.45 ± 0.03 s | x 4.91 |
| block.rb | 0.41 ± 0.01 s | 1.04 ± 0.02 s | x 2.55 |
| ao_bench.rb | 8.90 ± 0.09 s | 31.21 ± 0.93 s | x 3.51 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.8K | 11.6K | x 0.84 |
| app_mandelbrot.rb | 0.0M | 1.8M | x 127.05 |
| fibo.rb | 13.6K | 4.7K | x 0.34 |
| block.rb | 13.6K | 4.6K | x 0.34 |
| ao_bench.rb | 0.0M | 4.5M | x 313.80 |
