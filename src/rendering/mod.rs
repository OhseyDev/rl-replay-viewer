use ash::khr::swapchain;
use ash::vk::{PhysicalDevice, PhysicalDeviceType, SwapchainCreateInfoKHR, SwapchainKHR};
use ash::{vk, Instance};
use std::alloc;
use std::alloc::{alloc, Layout};
use std::ffi::{CStr, CString, NulError};
use std::os::raw::c_char;
use std::str::FromStr;
use winit::error::OsError;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowAttributes};

#[derive(Debug)]
pub enum VulkanError {
    Loading(ash::LoadingError),
    Error(vk::Result),
    NoSuitableDevice,
    UnableToFindQueueFamily,
    NulError(NulError),
    WindowHandleError(HandleError),
    OsError(OsError),
}

impl From<ash::LoadingError> for VulkanError {
    fn from(err: ash::LoadingError) -> Self {
        Self::Loading(err)
    }
}

impl From<vk::Result> for VulkanError {
    fn from(err: vk::Result) -> Self {
        Self::Error(err)
    }
}

impl From<NulError> for VulkanError {
    fn from(err: NulError) -> Self {
        Self::NulError(err)
    }
}

impl From<HandleError> for VulkanError {
    fn from(err: HandleError) -> Self {
        Self::WindowHandleError(err)
    }
}

impl From<OsError> for VulkanError {
    fn from(err: OsError) -> Self {
        Self::OsError(err)
    }
}

pub struct WindowView {
    window: Window,
    surface: vk::SurfaceKHR,
    swapchain: SwapchainKHR,
}

pub struct Device {
    entry: ash::Entry,
    instance: Instance,
    logical: ash::Device,
    physical: PhysicalDevice,
    swapchain_loader: swapchain::Device,
}

impl WindowView {
    pub fn new(event_loop: &ActiveEventLoop, device: &Device) -> Result<Self, VulkanError> {
        let attribs = WindowAttributes::default().with_title("RL Replay Viewer");
        let window = event_loop.create_window(attribs)?;
        let (surface, swapchain) = device.link_to_window(&window)?;
        Ok(Self {
            window,
            surface,
            swapchain,
        })
    }
}

impl Device {
    pub fn new(
        event_loop: &EventLoop<()>,
        extensions_instance: Vec<&str>,
        extensions_device: Vec<&str>,
    ) -> Result<Self, VulkanError> {
        let (instance_ext, device_ext) = unsafe {
            Self::efficiently_handle_extensions(event_loop, extensions_instance, extensions_device)?
        };
        #[cfg(debug_assertions)]
        unsafe {
            let mut extensions: Vec<String> = instance_ext
                .iter()
                .map(|&c| CStr::from_ptr(c.clone()).to_string_lossy().to_string())
                .collect();
            println!("Instance Extensions: {:?}", extensions);
            extensions = device_ext
                .iter()
                .map(|&c| CStr::from_ptr(c.clone()).to_string_lossy().to_string())
                .collect();
            println!("Device Extensions: {:?}", extensions);
        }
        let (entry, instance) = unsafe { Self::init(&instance_ext)? };
        let physical = unsafe { Self::pick_physical(&instance)? };
        let queue_family_index =
            if let Some(u) = unsafe { Self::find_queue_family_index(&instance, physical) } {
                u
            } else {
                return Err(VulkanError::UnableToFindQueueFamily);
            };
        let logical =
            unsafe { Self::create_logical(&instance, physical, &device_ext, queue_family_index)? };
        let swapchain_loader = swapchain::Device::new(&instance, &logical);
        Ok(Self {
            entry,
            instance,
            logical,
            physical,
            swapchain_loader,
        })
    }

    #[inline]
    unsafe fn efficiently_handle_extensions(
        event_loop: &EventLoop<()>,
        instance: Vec<&str>,
        device: Vec<&str>,
    ) -> Result<(Box<[*const c_char]>, Box<[*const c_char]>), VulkanError> {
        let instance_required =
            ash_window::enumerate_required_extensions(event_loop.display_handle()?.as_raw())?;
        let device_required = &[swapchain::NAME.as_ptr()];
        let calculate_length = |cs: &Vec<&str>, cp: &[*const c_char]| cs.len() + cp.len();
        let l_inst_cp = calculate_length(&instance, instance_required);
        let l_dev_cp = calculate_length(&device, device_required);
        let total_len = l_inst_cp + l_dev_cp;
        println!("total_len: {}", total_len);
        let layout =
            Layout::from_size_align(total_len * size_of::<&str>(), align_of::<*const c_char>())
                .expect("Incorrect alignment");
        let raw = alloc(layout) as *mut *const c_char;
        let inst_cp = std::slice::from_raw_parts_mut(raw, l_inst_cp - 1);
        let dev_cp = std::slice::from_raw_parts_mut(raw.add(l_inst_cp), l_dev_cp - 1);

        let append = |dest: &mut [*const c_char], src1: Vec<&str>, src2: &[*const c_char]| {
            let mut i = 0;
            src1.iter().for_each(|&s| {
                println!("{}", s);
                dest[i] = CString::from_str(s)
                    .expect("CString error")
                    .as_c_str()
                    .as_ptr();
                i += 1;
            });
            // Can't implement the following loop using Rust Iterator's as there's no built-in size to the slice.
            // Learnt the hard way and spent an hour and a half debugging the memory access violation.
            while i < dest.len() {
                dest[i] = src2[i - src1.len()];
                i += 1;
            }
        };
        append(inst_cp, instance, instance_required);
        append(dev_cp, device, device_required);
        Ok((Box::from_raw(inst_cp), Box::from_raw(dev_cp)))
    }
    #[inline]
    unsafe fn pick_physical(instance: &Instance) -> Result<PhysicalDevice, VulkanError> {
        let mut selected = None;
        let all = instance.enumerate_physical_devices()?;
        for physical_device in all {
            let properties = instance.get_physical_device_properties(physical_device);
            selected = match properties.device_type {
                PhysicalDeviceType::VIRTUAL_GPU
                | PhysicalDeviceType::DISCRETE_GPU
                | PhysicalDeviceType::INTEGRATED_GPU => {
                    Self::pick_between(instance, &properties, physical_device, selected)
                }
                _ => continue,
            }
        }
        if let Some(selected) = selected {
            Ok(selected)
        } else {
            Err(VulkanError::NoSuitableDevice)
        }
    }

    #[inline]
    unsafe fn pick_between(
        instance: &Instance,
        properties: &vk::PhysicalDeviceProperties,
        this: PhysicalDevice,
        other: Option<PhysicalDevice>,
    ) -> Option<PhysicalDevice> {
        let dev = if let Some(dev) = other {
            dev
        } else {
            return Some(this);
        };
        let properties_o = instance.get_physical_device_properties(dev);
        let d = if properties.device_type == PhysicalDeviceType::INTEGRATED_GPU
            && properties_o.device_type == PhysicalDeviceType::VIRTUAL_GPU
        {
            this
        } else if properties.device_type == PhysicalDeviceType::DISCRETE_GPU
            && (properties_o.device_type == PhysicalDeviceType::VIRTUAL_GPU
                || properties_o.device_type == PhysicalDeviceType::INTEGRATED_GPU)
        {
            this
        } else {
            dev
        };
        Some(d)
    }
    #[inline]
    unsafe fn init(extensions: &[*const c_char]) -> Result<(ash::Entry, Instance), VulkanError> {
        let entry = ash::Entry::load()?;
        let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_0);
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extensions);
        let instance = entry.create_instance(&create_info, None)?;
        Ok((entry, instance))
    }

    #[inline]
    unsafe fn create_logical(
        instance: &Instance,
        physical: PhysicalDevice,
        extensions: &[*const c_char],
        queue_family_index: u32,
    ) -> Result<ash::Device, VulkanError> {
        let features = vk::PhysicalDeviceFeatures::default();
        let queue_info =
            vk::DeviceQueueCreateInfo::default().queue_family_index(queue_family_index);
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(extensions)
            .enabled_features(&features);
        Ok(instance.create_device(physical, &create_info, None)?)
    }

    #[inline]
    unsafe fn find_queue_family_index(
        instance: &Instance,
        physical: PhysicalDevice,
    ) -> Option<u32> {
        let queue_families = instance.get_physical_device_queue_family_properties(physical);
        let mut index = 0;
        for queue_family in queue_families {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    // Surface and swapchain stuff

    pub fn link_to_window(
        &self,
        window: &Window,
    ) -> Result<(vk::SurfaceKHR, SwapchainKHR), VulkanError> {
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        // TODO: Handle queue families properly instead of doing it "lazily" and definitely incorrectly.
        let create_info = SwapchainCreateInfoKHR::default().surface(surface);
        let swapchain = unsafe { self.swapchain_loader.create_swapchain(&create_info, None)? };
        Ok((surface, swapchain))
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.logical.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
