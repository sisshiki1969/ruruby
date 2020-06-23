# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.71 ± 0.01 s | 2.58 ± 0.02 s  | x 1.51 |
| app_mandelbrot.rb | 2.26 ± 0.15 s | 5.89 ± 0.08 s  | x 2.61 |
|      fibo.rb      | 0.45 ± 0.01 s | 1.89 ± 0.02 s  | x 4.21 |
|     block.rb      | 0.36 ± 0.00 s | 0.95 ± 0.03 s  | x 2.62 |
|    ao_bench.rb    | 9.08 ± 0.08 s | 26.04 ± 0.28 s | x 2.87 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 47.83 ± 0.85 fps  | 8.47 ± 0.12 fps  | x 5.64 |
| optcarrot --opt | 144.38 ± 3.71 fps | 39.49 ± 0.99 fps | x 3.66 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  4.7K  | x 0.34  |
| app_mandelbrot.rb | 14.0K |  6.4K  | x 0.46  |
|      fibo.rb      | 13.8K |  4.7K  | x 0.34  |
|     block.rb      | 13.8K |  4.6K  | x 0.33  |
|    ao_bench.rb    | 14.4K |  7.2K  | x 0.50  |
|     optcarrot     | 68.5K | 81.2K  | x 1.18  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 17.56 |
