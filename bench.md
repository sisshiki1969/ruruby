# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.79 ± 0.05 s | 2.45 ± 0.03 s  | x 1.37 |
| app_mandelbrot.rb | 2.18 ± 0.04 s | 5.92 ± 0.07 s  | x 2.71 |
|    app_fibo.rb    | 0.45 ± 0.00 s | 1.95 ± 0.02 s  | x 4.29 |
|     block.rb      | 0.38 ± 0.01 s | 1.02 ± 0.01 s  | x 2.67 |
|  app_aobench.rb   | 9.20 ± 0.06 s | 25.74 ± 0.23 s | x 2.80 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 48.80 ± 1.79 fps  | 9.30 ± 0.08 fps  | x 5.25 |
| optcarrot --opt | 141.66 ± 3.79 fps | 40.52 ± 0.96 fps | x 3.50 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  5.1K  | x 0.36  |
| app_mandelbrot.rb | 14.0K |  6.7K  | x 0.48  |
|    app_fibo.rb    | 13.7K |  4.9K  | x 0.36  |
|     block.rb      | 13.8K |  4.9K  | x 0.35  |
|  app_aobench.rb   | 14.5K |  7.5K  | x 0.52  |
|     optcarrot     | 68.4K | 81.4K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.7M  | x 19.56 |
