use core::ffi::CStr;

use log::info;

const SDMMC_HOST_FLAG_SPI: u32 = 1 << 3;
const SDMMC_HOST_FLAG_DEINIT_ARG: u32 = 1 << 5;

pub fn mount_sdcard(mount_point: &CStr) -> anyhow::Result<()> {
    use esp_idf_hal::sys::*;

    info!("SD: building host config...");

    let host = sdmmc_host_t {
        flags: SDMMC_HOST_FLAG_SPI | SDMMC_HOST_FLAG_DEINIT_ARG,
        slot: spi_host_device_t_SPI2_HOST as i32,
        max_freq_khz: SDMMC_FREQ_DEFAULT as i32,
        io_voltage: 3.3,
        init: Some(sdspi_host_init),
        set_card_clk: Some(sdspi_host_set_card_clk),
        do_transaction: Some(sdspi_host_do_transaction),
        __bindgen_anon_1: sdmmc_host_t__bindgen_ty_1 {
            deinit_p: Some(sdspi_host_remove_device),
        },
        io_int_enable: Some(sdspi_host_io_int_enable),
        io_int_wait: Some(sdspi_host_io_int_wait),
        command_timeout_ms: 10000,
        get_real_freq: Some(sdspi_host_get_real_freq),
        ..Default::default()
    };

    let slot_config = sdspi_device_config_t {
        host_id: spi_host_device_t_SPI2_HOST,
        gpio_cs: 22,
        gpio_cd: -1,
        gpio_wp: -1,
        gpio_wp_polarity: false,
        gpio_int: -1,
    };

    let mount_config = esp_vfs_fat_mount_config_t {
        format_if_mount_failed: false,
        max_files: 5,
        allocation_unit_size: 0,
        disk_status_check_enable: false,
    };

    info!("SD: calling esp_vfs_fat_sdspi_mount...");

    let mut card: *mut sdmmc_card_t = core::ptr::null_mut();

    // SAFETY: esp_vfs_fat_sdspi_mount is a C FFI call. All pointer arguments
    // point to valid stack-allocated structs within this function's scope.
    // The card handle is initialized by the IDF on success. SPI host config
    // is fully populated to avoid null-pointer deref in get_real_freq.
    let ret = unsafe {
        esp_vfs_fat_sdspi_mount(
            mount_point.as_ptr(),
            &host,
            &slot_config,
            &mount_config,
            &mut card,
        )
    };

    info!("SD: esp_vfs_fat_sdspi_mount returned {:x}", ret as u32);

    if ret != ESP_OK {
        anyhow::bail!("esp_vfs_fat_sdspi_mount failed: {:#x}", ret as u32);
    }

    // SAFETY: card was just validated by a successful esp_vfs_fat_sdspi_mount
    // call, which guarantees it points to a valid sdmmc_card_t.
    let card_ref = unsafe { &*card };
    let size_mb = card_ref.csd.capacity as u64 * 512 / 1024 / 1024;
    info!("SD card mounted, size={}MB", size_mb);

    Ok(())
}
