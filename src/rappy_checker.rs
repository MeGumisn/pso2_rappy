use opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT;
use opencv::core::{
    add_weighted, divide2, mean, multiply, no_array, subtract, Mat,
    MatTraitConst, Point, Size, BORDER_DEFAULT, CV_32F,
};
use opencv::imgproc;
use opencv::imgproc::cvt_color;
use std::cell::RefCell;
use std::ops::Deref;
use log::{debug, info};

pub fn get_threshold_mat(mat: &Mat, thresh: u8) -> Mat {
    let gray;
    let mut gray_mat;
    let channels = mat.channels();
    if channels==3||channels==4 {
        let cvt_code;
        if channels == 4 {
            cvt_code = imgproc::COLOR_BGRA2GRAY;
        } else {
            cvt_code = imgproc::COLOR_BGR2GRAY;
        }
        gray_mat = Mat::default();
        let _ = cvt_color(
            mat,
            &mut gray_mat,
            cvt_code,
            0,
            ALGO_HINT_DEFAULT,
        );
        gray = &gray_mat;
    }else{
        gray = mat;
    }
    let mut thresh_mat = Mat::default();
    let _ = imgproc::threshold(
        &gray,
        &mut thresh_mat,
        thresh as f64,
        255.0,
        imgproc::THRESH_OTSU,
    );
    thresh_mat
}

/// 计算两个 Mat 之间的 SSIM
/// 返回 Scalar channel 1或者channel 1 2 3的均值，因为 SSIM 是分通道计算的（BGR 每个通道一个值）
/// 仿C++版本, unsafe部分是为了绕过&var和&mut var无法同时使用的问题
///
/// ## Overloaded Parameters
///
/// ## Parameters
///
/// * img1: opencv mat.
/// * img2: opencv mat.
/// * is_gray: mat.channels() == 1.
pub fn get_ssim(img1: &Mat, img2: &Mat, is_single_channel: bool) -> opencv::Result<f64> {
    if cfg!(debug_assertions){
        info!(
                    "images channel: {} vs {}, rows: {} vs {}, cols: {} vs {}",
                    img1.channels(),img2.channels(),
                    img1.rows(),img2.rows(),
                    img1.cols(),img2.cols()
                );
    }
    let c1 = 6.5025; // (0.01 * 255)^2
    let c2 = 58.5225; // (0.03 * 255)^2

    let mut i1 = Mat::default();
    let mut i2 = Mat::default();
    // 转换为 32 位浮点数以保证计算精度
    img1.convert_to(&mut i1, CV_32F, 1.0, 0.0)?;
    img2.convert_to(&mut i2, CV_32F, 1.0, 0.0)?;

    let mut i1_2 = Mat::default();
    let mut i2_2 = Mat::default();
    let mut i1_i2 = Mat::default();

    multiply(&i1, &i1, &mut i1_2, 1.0, -1)?; // i1^2
    multiply(&i2, &i2, &mut i2_2, 1.0, -1)?; // i2^2
    multiply(&i1, &i2, &mut i1_i2, 1.0, -1)?; // i1 * i2

    // 均值计算 (μ)
    let mut mu1 = Mat::default();
    let mut mu2 = Mat::default();
    let ksize = Size::new(7, 7);
    let anchor = Point { x: -1, y: -1 };
    imgproc::blur(&i1, &mut mu1, ksize, anchor, BORDER_DEFAULT)?;
    imgproc::blur(&i2, &mut mu2, ksize, anchor, BORDER_DEFAULT)?;

    let mut mu1_2 = Mat::default();
    let mut mu2_2 = Mat::default();
    let mut mu1_mu2 = Mat::default();
    multiply(&mu1, &mu1, &mut mu1_2, 1.0, -1)?; // μ1^2
    multiply(&mu2, &mu2, &mut mu2_2, 1.0, -1)?; // μ2^2
    multiply(&mu1, &mu2, &mut mu1_mu2, 1.0, -1)?; // μ1 * μ2

    // 方差与协方差计算 (σ)
    let sigma1_2 = RefCell::new(Mat::default());
    let sigma2_2 = RefCell::new(Mat::default());
    let sigma12 = RefCell::new(Mat::default());
    let mut_sigma1_2 = unsafe { &mut *sigma1_2.as_ptr() };
    let mut_sigma2_2 = unsafe { &mut *sigma2_2.as_ptr() };
    let mut_sigma12 = unsafe { &mut *sigma12.as_ptr() };
    imgproc::blur(&i1_2, mut_sigma1_2, ksize, anchor, BORDER_DEFAULT)?;
    subtract(
        sigma1_2.borrow().deref(),
        &mu1_2,
        mut_sigma1_2,
        &no_array(),
        -1,
    )?; // σ1^2 = E[i1^2] - μ1^2

    imgproc::blur(&i2_2, mut_sigma2_2, ksize, anchor, BORDER_DEFAULT)?;
    subtract(
        sigma2_2.borrow().deref(),
        &mu2_2,
        mut_sigma2_2,
        &no_array(),
        -1,
    )?; // σ2^2 = E[i2^2] - μ2^2

    imgproc::blur(&i1_i2, mut_sigma12, ksize, anchor, BORDER_DEFAULT)?;
    subtract(
        &sigma12.borrow().deref(),
        &mu1_mu2,
        mut_sigma12,
        &no_array(),
        -1,
    )?; // σ12 = E[i1*i2] - μ1*μ2

    // 公式: SSIM = ((2*mu1*mu2 + C1) * (2*sigma12 + C2)) / ((mu1^2 + mu2^2 + C1) * (sigma1^2 + sigma2^2 + C2))
    let t1 = RefCell::new(Mat::default()); // 2*mu1*mu2 + C1
    let t2 = RefCell::new(Mat::default()); // 2*sigma12 + C2
    let t3 = RefCell::new(Mat::default()); // t1 * t2

    let mut_t1 = unsafe { &mut *t1.as_ptr() };
    let mut_t2 = unsafe { &mut *t2.as_ptr() };
    let mut_t3 = unsafe { &mut *t3.as_ptr() };

    // t1 = 2 * mu1_mu2 + C1;
    let _ = &mu1_mu2.convert_to(mut_t1, CV_32F, 2.0, c1)?;
    // t2 = 2 * sigma12 + C2;
    sigma12.borrow().convert_to(mut_t2, CV_32F, 2.0, c2)?;
    // t3 = t1*t2
    multiply(t1.borrow().deref(), t2.borrow().deref(), mut_t3, 1.0, -1)?;
    // t1 = mu1_2 + mu2_2 + C1;
    add_weighted(&mu1_2, 1.0, &mu2_2, 1.0, c1, mut_t1, -1)?;
    // t2 = sigma1_2 + sigma2_2 + C2;
    add_weighted(
        sigma1_2.borrow().deref(),
        1.0,
        sigma2_2.borrow().deref(),
        1.0,
        c2,
        mut_t2,
        -1,
    )?;

    multiply(t1.borrow().deref(), t2.borrow().deref(), mut_t1, 1.0, -1)?;

    let mut ssim_map = Mat::default();
    // ssim_map =  t3./t1
    divide2(
        t3.borrow().deref(),
        t1.borrow().deref(),
        &mut ssim_map,
        1.0,
        -1,
    )?;

    // 对全图取平均值
    // mssim = average of ssim map
    let mssim = mean(&ssim_map, &no_array())?;
    if is_single_channel {
        if cfg!(debug_assertions) {
           debug!("Sim: {:?}", mssim);
        };
        Ok(mssim[0])
    } else {
        if cfg!(debug_assertions){
            debug!("Sim: {:?}", mssim.0);
        };
        Ok(mssim[0]+mssim[1]+mssim[2] / 3f64)
    }
}