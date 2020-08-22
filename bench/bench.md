# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| block.rb | 0.26 ± 0.02 s | 0.49 ± 0.02 s | x 1.85 |
| for_loop.rb | 0.32 ± 0.01 s | 0.42 ± 0.00 s | x 1.31 |
| so_mandelbrot.rb | 1.98 ± 0.06 s | 2.31 ± 0.09 s | x 1.17 |
| app_mandelbrot.rb | 1.18 ± 0.01 s | 1.10 ± 0.01 s | x 0.93 |
| app_fibo.rb | 0.52 ± 0.00 s | 1.70 ± 0.01 s | x 3.26 |
| app_aobench.rb | 10.60 ± 0.07 s | 20.40 ± 0.10 s | x 1.92 |

## optcarrot benchmark

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| optcarrot  | 41.99 ± 1.92 fps | 13.50 ± 0.39 fps | x 3.11 |
| optcarrot --opt | 135.45 ± 0.90 fps | 53.62 ± 0.49 fps | x 2.53 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| block.rb | 21.8M | 4.6M | x 0.21 |
| for_loop.rb | 21.9M | 4.6M | x 0.21 |
| so_mandelbrot.rb | 22.2M | 5.8M | x 0.26 |
| app_mandelbrot.rb | 22.2M | 5.5M | x 0.25 |
| app_fibo.rb | 21.9M | 4.7M | x 0.21 |
| app_aobench.rb | 22.6M | 7.9M | x 0.35 |
| optcarrot  | 76.7M | 64.3M | x 0.84 |
| optcarrot --opt | 94.1M | 1027.5M | x 10.92 |
