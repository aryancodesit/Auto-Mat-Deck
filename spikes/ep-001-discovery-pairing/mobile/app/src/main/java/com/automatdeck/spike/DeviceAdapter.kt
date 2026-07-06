package com.automatdeck.spike

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView

class DeviceAdapter(
    private val devices: List<DiscoveredDevice>,
    private val onClick: (DiscoveredDevice) -> Unit
) : RecyclerView.Adapter<DeviceAdapter.ViewHolder>() {

    inner class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val nameText: TextView = view.findViewById(android.R.id.text1)
        val detailText: TextView = view.findViewById(android.R.id.text2)

        init {
            view.setOnClickListener {
                view.tag?.let { onClick(it as DiscoveredDevice) }
            }
        }
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val view = LayoutInflater.from(parent.context)
            .inflate(android.R.layout.simple_list_item_2, parent, false)
        return ViewHolder(view)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val device = devices[position]
        holder.nameText.text = device.name
        holder.detailText.text = "${device.host}:${device.port} | ${device.deviceId}"
        holder.itemView.tag = device
    }

    override fun getItemCount() = devices.size
}
