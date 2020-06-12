# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.78 ± 0.01 s | 2.92 ± 0.06 s | x 1.64 |
| app_mandelbrot.rb | 2.29 ± 0.05 s | 6.97 ± 0.03 s | x 3.05 |
| fibo.rb | 0.48 ± 0.00 s | 2.37 ± 0.03 s | x 4.97 |
| block.rb | 0.41 ± 0.00 s | 1.10 ± 0.04 s | x 2.66 |
| ao_bench.rb | 9.48 ± 0.11 s | 29.84 ± 0.96 s | x 3.15 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.9K | 4.8K | x 0.35 |
| app_mandelbrot.rb | 13.9K | 19.7K | x 1.42 |
| fibo.rb | 13.6K | 4.6K | x 0.34 |
| block.rb | 13.6K | 4.6K | x 0.34 |
| ao_bench.rb | 0.0M | 4.5M | x 316.49 |
