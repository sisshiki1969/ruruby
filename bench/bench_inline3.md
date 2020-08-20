# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 20.04 LTS

## execution time

|     benchmark     |      ruby      |     ruruby     |  rate  |
| :---------------: | :------------: | :------------: | :----: |
|     block.rb      | 0.25 ± 0.02 s  | 0.47 ± 0.02 s  | x 1.86 |
|    for_loop.rb    | 0.31 ± 0.00 s  | 0.41 ± 0.01 s  | x 1.33 |
| so_mandelbrot.rb  | 2.04 ± 0.03 s  | 2.25 ± 0.01 s  | x 1.10 |
| app_mandelbrot.rb | 1.16 ± 0.00 s  | 1.05 ± 0.01 s  | x 0.91 |
|    app_fibo.rb    | 0.52 ± 0.02 s  | 1.66 ± 0.02 s  | x 3.16 |
|  app_aobench.rb   | 10.47 ± 0.13 s | 20.09 ± 0.26 s | x 1.92 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 43.71 ± 0.21 fps  | 13.65 ± 0.10 fps | x 3.20 |
| optcarrot --opt | 135.00 ± 1.12 fps | 54.58 ± 2.94 fps | x 2.47 |

## memory consumption

|     benchmark     | ruby  | ruruby  |  rate   |
| :---------------: | :---: | :-----: | :-----: |
|     block.rb      | 21.9M |  4.6M   | x 0.21  |
|    for_loop.rb    | 21.9M |  4.6M   | x 0.21  |
| so_mandelbrot.rb  | 22.2M |  5.9M   | x 0.26  |
| app_mandelbrot.rb | 22.3M |  5.5M   | x 0.25  |
|    app_fibo.rb    | 21.9M |  4.7M   | x 0.21  |
|  app_aobench.rb   | 22.6M |  7.9M   | x 0.35  |
|     optcarrot     | 76.8M |  64.2M  | x 0.84  |
|  optcarrot --opt  | 94.0M | 1027.3M | x 10.93 |
