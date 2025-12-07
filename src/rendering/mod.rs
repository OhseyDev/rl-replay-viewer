use ash::{vk, Instance};
use ash::vk::PhysicalDevice;

#[derive(Debug)]
pub enum VulkanError {
    Loading(ash::LoadingError),
    Error(vk::Result),
    NoSuitableDevice,
    UnableToFindQueueFamily,
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

pub struct Device {
    entry: ash::Entry,
    instance: ash::Instance,
    logical: ash::Device,
    physical: vk::PhysicalDevice,
}

impl Device {
    pub fn new() -> Result<Self, VulkanError> {
        let (entry, instance) = unsafe { Self::init()? };
        let physical = unsafe { Self::pick_physical(&instance)? };
        let queue_family_index = if let Some(u) = unsafe { Self::find_queue_family_index(&instance, physical) } {
            u
        } else {
            return Err(VulkanError::UnableToFindQueueFamily);
        };
        let logical = unsafe { Self::create_logical(&instance, physical, queue_family_index)? };
        Ok(Self { entry, instance, logical, physical })
    }

    #[inline]
    unsafe fn pick_physical(instance: &ash::Instance) -> Result<vk::PhysicalDevice, VulkanError> {
        use vk::{PhysicalDevice, PhysicalDeviceType};
        let mut selected = None;
        let all = instance.enumerate_physical_devices()?;
        for physical_device in all {
            let properties = instance.get_physical_device_properties(physical_device);
            selected = match properties.device_type {
                PhysicalDeviceType::VIRTUAL_GPU
                | PhysicalDeviceType::DISCRETE_GPU
                | PhysicalDeviceType::INTEGRATED_GPU => Self::pick_between(instance, &properties, physical_device, selected),
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
    unsafe fn pick_between(instance: &ash::Instance, properties: &vk::PhysicalDeviceProperties, this: vk::PhysicalDevice, other: Option<vk::PhysicalDevice>) -> Option<vk::PhysicalDevice> {
        let dev = if let Some(dev) = other {
            dev
        } else {
            return Some(this);
        };
        let properties_o = instance.get_physical_device_properties(dev);
        let d = if properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU &&
            properties_o.device_type == vk::PhysicalDeviceType::VIRTUAL_GPU {
            this
        } else if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU &&
            (properties_o.device_type == vk::PhysicalDeviceType::VIRTUAL_GPU ||
                properties_o.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU)  {
            this
        } else {
            dev
        };
        Some(d)
    }
    #[inline]
    unsafe fn init() -> Result<(ash::Entry, ash::Instance), VulkanError> {
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };
        let entry = ash::Entry::load()?;
        let instance = entry.create_instance(&create_info, None)?;
        Ok((entry, instance))
    }

    #[inline]
    unsafe fn create_logical(instance: &Instance, physical: vk::PhysicalDevice, queue_family_index: u32) -> Result<ash::Device, VulkanError> {
        let features = vk::PhysicalDeviceFeatures::default();
        let queue_info = vk::DeviceQueueCreateInfo::default().queue_family_index(queue_family_index);
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_features(&features);
        Ok(instance.create_device(physical, &create_info, None)?)
    }

    #[inline]
    unsafe fn find_queue_family_index(instance: &Instance, physical: PhysicalDevice) -> Option<u32> {
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
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.logical.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
