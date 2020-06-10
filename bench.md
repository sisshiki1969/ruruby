# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.75 ± 0.02 s | 2.99 ± 0.05 s | x 1.71 |
| app_mandelbrot.rb | 2.31 ± 0.11 s | 6.66 ± 0.07 s | x 2.88 |
| fibo.rb | 0.47 ± 0.00 s | 2.45 ± 0.01 s | x 5.17 |
| block.rb | 0.42 ± 0.02 s | 1.07 ± 0.05 s | x 2.56 |
| ao_bench.rb | 9.29 ± 0.18 s | 30.09 ± 0.94 s | x 3.24 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.8K | 4.8K | x 0.35 |
| app_mandelbrot.rb | 13.9K | 6.0K | x 0.43 |
| fibo.rb | 13.6K | 4.7K | x 0.34 |
| block.rb | 13.6K | 4.6K | x 0.34 |
| ao_bench.rb | 0.0M | 4.5M | x 316.25 |
